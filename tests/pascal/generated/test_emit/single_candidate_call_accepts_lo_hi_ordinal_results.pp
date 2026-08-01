unit u;
interface
procedure take_word(w : word);
procedure take_cardinal(c : cardinal);
procedure run(i : longint; q : int64);
implementation
procedure take_word(w : word); begin end;
procedure take_cardinal(c : cardinal); begin end;
procedure run(i : longint; q : int64);
begin
  take_word(hi(i));
  take_word(lo(i));
  take_cardinal(hi(q));
  take_cardinal(lo(q));
end;
end.
