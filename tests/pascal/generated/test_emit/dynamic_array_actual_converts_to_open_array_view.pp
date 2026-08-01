unit u;
interface
type
  tints = array of longint;
procedure log(const xs : array of longint);
implementation
procedure log(const xs : array of longint);
begin
end;
procedure demo;
var
  ys : tints;
begin
  setlength(ys, 2);
  log(ys);
end;
end.
