unit u;
interface
procedure demo(raw : pointer);
implementation
procedure demo(raw : pointer);
type
  setbytes = array[0..31] of byte;
  psetbytes = ^setbytes;
begin
  writeln(psetbytes(raw)^[0]);
end;
end.
