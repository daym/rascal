unit u;
interface
type
  tbase = class
  end;
  tchild = class(tbase)
  end;
procedure take(var b : tbase);
procedure demo(c : tchild);
implementation
procedure take(var b : tbase);
begin
end;
procedure demo(c : tchild);
begin
  take(c);
end;
end.
