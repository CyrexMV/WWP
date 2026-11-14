-- Monteure gleichen auch die Nummernkreise ab (keine doppelten Protokollnummern)
create or replace function privat.monteur_typ(t text) returns boolean
language sql immutable set search_path = '' as $$
  select t in ('kunden', 'anlagen', 'wartungen', 'termine', 'firma', 'zaehler')
$$;
