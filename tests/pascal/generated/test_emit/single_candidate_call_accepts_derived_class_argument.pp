unit u;
interface
type
  tbase = class end;
  tchild = class(tbase) end;
procedure take(o : tbase);
procedure run(c : tchild);
implementation
procedure take(o : tbase); begin end;
procedure run(c : tchild);
begin
  take(c);
end;
end.
