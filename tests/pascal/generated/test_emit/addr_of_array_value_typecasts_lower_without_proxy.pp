unit u;
interface
type
  tbuf = array[0..3] of char;
  plongint = ^longint;
procedure demo;
implementation
procedure demo;
var
  buf : tbuf;
  raw : pointer;
  bits : ptrint;
  ints : plongint;
begin
  raw := pointer(@buf);
  bits := ptrint(@buf);
  ints := plongint(@buf);
end;
end.
