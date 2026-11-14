-- Betriebe (Mandanten), Mitglieder mit Rolle, Einladungen, synchronisierte Einträge, Geräte

create table public.betriebe (
  id uuid primary key default gen_random_uuid(),
  name text not null default '',
  erstellt_am timestamptz not null default now()
);

create table public.mitglieder (
  betrieb_id uuid not null references public.betriebe(id) on delete cascade,
  user_id uuid not null references auth.users(id) on delete cascade,
  rolle text not null check (rolle in ('chef', 'monteur')),
  name text not null default '',
  email text not null default '',
  beigetreten_am timestamptz not null default now(),
  primary key (betrieb_id, user_id)
);
create index mitglieder_user_idx on public.mitglieder(user_id);

create table public.einladungen (
  code text primary key,
  betrieb_id uuid not null references public.betriebe(id) on delete cascade,
  rolle text not null check (rolle in ('chef', 'monteur')),
  erstellt_von uuid not null references auth.users(id) on delete cascade,
  erstellt_am timestamptz not null default now(),
  gueltig_bis timestamptz not null default now() + interval '7 days',
  eingeloest_von uuid references auth.users(id) on delete set null
);
create index einladungen_betrieb_idx on public.einladungen(betrieb_id);

-- Jeder Datensatz der App (Kunde, Dokument, Buchung …) als JSON, je Betrieb
create table public.eintraege (
  betrieb_id uuid not null references public.betriebe(id) on delete cascade,
  typ text not null,
  id bigint not null,
  daten jsonb not null default '{}'::jsonb,
  geloescht boolean not null default false,
  geaendert timestamptz not null default now(),
  geaendert_von uuid references auth.users(id) on delete set null,
  primary key (betrieb_id, typ, id)
);
create index eintraege_sync_idx on public.eintraege(betrieb_id, geaendert);

-- Geräte: eindeutige Nummer je Gerät für kollisionsfreie IDs
create table public.geraete (
  nr bigint generated always as identity primary key,
  betrieb_id uuid not null references public.betriebe(id) on delete cascade,
  user_id uuid not null references auth.users(id) on delete cascade,
  angemeldet_am timestamptz not null default now()
);
create index geraete_betrieb_idx on public.geraete(betrieb_id);

-- Änderungszeitpunkt immer vom Server
create or replace function public.eintraege_stempel() returns trigger
language plpgsql set search_path = '' as $$
begin
  new.geaendert := now();
  new.geaendert_von := auth.uid();
  return new;
end $$;
create trigger eintraege_stempel before insert or update on public.eintraege
for each row execute function public.eintraege_stempel();

-- Hilfsfunktionen für die Zugriffsregeln (security definer, damit RLS nicht rekursiv prüft)
create or replace function public.rolle_in(b uuid) returns text
language sql stable security definer set search_path = '' as $$
  select m.rolle from public.mitglieder m where m.betrieb_id = b and m.user_id = auth.uid()
$$;

-- Was ein Monteur sehen und ändern darf (keine Preise, Rechnungen, Finanzen)
create or replace function public.monteur_typ(t text) returns boolean
language sql immutable set search_path = '' as $$
  select t in ('kunden', 'anlagen', 'wartungen', 'termine', 'firma')
$$;

alter table public.betriebe enable row level security;
alter table public.mitglieder enable row level security;
alter table public.einladungen enable row level security;
alter table public.eintraege enable row level security;
alter table public.geraete enable row level security;

create policy "Mitglieder sehen ihren Betrieb" on public.betriebe
  for select to authenticated using (public.rolle_in(id) is not null);
create policy "Chef ändert den Betrieb" on public.betriebe
  for update to authenticated using (public.rolle_in(id) = 'chef') with check (public.rolle_in(id) = 'chef');

create policy "Mitglieder sehen das Team" on public.mitglieder
  for select to authenticated using (public.rolle_in(betrieb_id) is not null);
create policy "Chef entfernt Mitglieder" on public.mitglieder
  for delete to authenticated using (public.rolle_in(betrieb_id) = 'chef' and user_id <> auth.uid());

create policy "Chef sieht Einladungen" on public.einladungen
  for select to authenticated using (public.rolle_in(betrieb_id) = 'chef');
create policy "Chef löscht Einladungen" on public.einladungen
  for delete to authenticated using (public.rolle_in(betrieb_id) = 'chef');

create policy "Einträge lesen" on public.eintraege
  for select to authenticated
  using (public.rolle_in(betrieb_id) = 'chef' or (public.rolle_in(betrieb_id) = 'monteur' and public.monteur_typ(typ)));
create policy "Einträge anlegen" on public.eintraege
  for insert to authenticated
  with check (public.rolle_in(betrieb_id) = 'chef' or (public.rolle_in(betrieb_id) = 'monteur' and public.monteur_typ(typ) and typ <> 'firma'));
create policy "Einträge ändern" on public.eintraege
  for update to authenticated
  using (public.rolle_in(betrieb_id) = 'chef' or (public.rolle_in(betrieb_id) = 'monteur' and public.monteur_typ(typ) and typ <> 'firma'))
  with check (public.rolle_in(betrieb_id) = 'chef' or (public.rolle_in(betrieb_id) = 'monteur' and public.monteur_typ(typ) and typ <> 'firma'));

create policy "Chef sieht Geräte" on public.geraete
  for select to authenticated using (public.rolle_in(betrieb_id) = 'chef');

-- Betrieb anlegen: der Anlegende wird Chef
create or replace function public.betrieb_anlegen(firmenname text, mein_name text default '')
returns uuid language plpgsql security definer set search_path = '' as $$
declare b uuid;
begin
  if auth.uid() is null then raise exception 'nicht angemeldet'; end if;
  insert into public.betriebe(name) values (coalesce(firmenname, '')) returning id into b;
  insert into public.mitglieder(betrieb_id, user_id, rolle, name, email)
  values (b, auth.uid(), 'chef', coalesce(mein_name, ''), coalesce((select u.email from auth.users u where u.id = auth.uid()), ''));
  return b;
end $$;

-- Einladungscode (8 Zeichen, ohne verwechselbare Zeichen) – nur der Chef
create or replace function public.einladung_erstellen(b uuid, neue_rolle text default 'monteur')
returns text language plpgsql security definer set search_path = '' as $$
declare c text; zeichen text := 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'; i int;
begin
  if public.rolle_in(b) is distinct from 'chef' then raise exception 'nur der Chef kann einladen'; end if;
  if neue_rolle not in ('chef', 'monteur') then raise exception 'unbekannte Rolle'; end if;
  loop
    c := '';
    for i in 1..8 loop
      c := c || substr(zeichen, 1 + floor(random() * length(zeichen))::int, 1);
    end loop;
    exit when not exists (select 1 from public.einladungen e where e.code = c);
  end loop;
  insert into public.einladungen(code, betrieb_id, rolle, erstellt_von) values (c, b, neue_rolle, auth.uid());
  return c;
end $$;

-- Einladung einlösen
create or replace function public.einladung_einloesen(einladungscode text, mein_name text default '')
returns uuid language plpgsql security definer set search_path = '' as $$
declare e public.einladungen%rowtype;
begin
  if auth.uid() is null then raise exception 'nicht angemeldet'; end if;
  select * into e from public.einladungen x where x.code = upper(trim(einladungscode)) for update;
  if not found or e.eingeloest_von is not null or e.gueltig_bis < now() then
    raise exception 'Einladungscode ungültig oder abgelaufen';
  end if;
  insert into public.mitglieder(betrieb_id, user_id, rolle, name, email)
  values (e.betrieb_id, auth.uid(), e.rolle, coalesce(mein_name, ''), coalesce((select u.email from auth.users u where u.id = auth.uid()), ''))
  on conflict (betrieb_id, user_id) do update set rolle = excluded.rolle;
  update public.einladungen set eingeloest_von = auth.uid() where code = e.code;
  return e.betrieb_id;
end $$;

-- Gerät anmelden → eindeutige Gerätenummer
create or replace function public.geraet_anmelden(b uuid)
returns bigint language plpgsql security definer set search_path = '' as $$
declare n bigint;
begin
  if public.rolle_in(b) is null then raise exception 'kein Mitglied'; end if;
  insert into public.geraete(betrieb_id, user_id) values (b, auth.uid()) returning nr into n;
  return n;
end $$;

revoke execute on function public.betrieb_anlegen(text, text) from public, anon;
revoke execute on function public.einladung_erstellen(uuid, text) from public, anon;
revoke execute on function public.einladung_einloesen(text, text) from public, anon;
revoke execute on function public.geraet_anmelden(uuid) from public, anon;
revoke execute on function public.rolle_in(uuid) from public, anon;
grant execute on function public.betrieb_anlegen(text, text) to authenticated;
grant execute on function public.einladung_erstellen(uuid, text) to authenticated;
grant execute on function public.einladung_einloesen(text, text) to authenticated;
grant execute on function public.geraet_anmelden(uuid) to authenticated;
grant execute on function public.rolle_in(uuid) to authenticated;

-- Dateien (Logo, Belege, Fotos): Pfad = <betrieb>/<art>/<datei-id>
insert into storage.buckets (id, name, public) values ('dateien', 'dateien', false) on conflict (id) do nothing;

create policy "Dateien lesen" on storage.objects for select to authenticated
using (
  bucket_id = 'dateien'
  and (
    public.rolle_in(((storage.foldername(name))[1])::uuid) = 'chef'
    or (public.rolle_in(((storage.foldername(name))[1])::uuid) = 'monteur' and (storage.foldername(name))[2] in ('logo', 'foto'))
  )
);
create policy "Dateien hochladen" on storage.objects for insert to authenticated
with check (
  bucket_id = 'dateien'
  and (
    public.rolle_in(((storage.foldername(name))[1])::uuid) = 'chef'
    or (public.rolle_in(((storage.foldername(name))[1])::uuid) = 'monteur' and (storage.foldername(name))[2] = 'foto')
  )
);
create policy "Dateien ersetzen" on storage.objects for update to authenticated
using (bucket_id = 'dateien' and public.rolle_in(((storage.foldername(name))[1])::uuid) = 'chef');
create policy "Dateien löschen" on storage.objects for delete to authenticated
using (bucket_id = 'dateien' and public.rolle_in(((storage.foldername(name))[1])::uuid) = 'chef');
