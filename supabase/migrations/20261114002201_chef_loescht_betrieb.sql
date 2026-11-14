-- Der Chef darf seinen Betrieb löschen (samt aller Einträge, Mitglieder, Einladungen, Geräte)
create policy "Chef löscht den Betrieb" on public.betriebe
  for delete to authenticated using (privat.rolle_in(id) = 'chef');
