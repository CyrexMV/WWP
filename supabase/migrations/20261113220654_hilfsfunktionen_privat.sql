-- Hilfsfunktionen nicht über die API erreichbar machen
create schema if not exists privat;
revoke all on schema privat from public, anon;
grant usage on schema privat to authenticated;
alter function public.rolle_in(uuid) set schema privat;
alter function public.monteur_typ(text) set schema privat;
alter function privat.rolle_in(uuid) set search_path = '';

create or replace function public.einladung_erstellen(b uuid, neue_rolle text default 'monteur')
returns text language plpgsql security definer set search_path = '' as $$
declare c text; zeichen text := 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'; i int;
begin
  if privat.rolle_in(b) is distinct from 'chef' then raise exception 'nur der Chef kann einladen'; end if;
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

create or replace function public.geraet_anmelden(b uuid)
returns bigint language plpgsql security definer set search_path = '' as $$
declare n bigint;
begin
  if privat.rolle_in(b) is null then raise exception 'kein Mitglied'; end if;
  insert into public.geraete(betrieb_id, user_id) values (b, auth.uid()) returning nr into n;
  return n;
end $$;
