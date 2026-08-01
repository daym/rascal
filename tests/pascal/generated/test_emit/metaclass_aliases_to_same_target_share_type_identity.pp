unit u;
interface
type
  tfoo = class
  end;
  ta = class of tfoo;
  tb = class of tfoo;
procedure take(var c : ta);
var
  b : tb;
implementation
procedure take(var c : ta);
begin
end;
begin
  take(b);
end.
