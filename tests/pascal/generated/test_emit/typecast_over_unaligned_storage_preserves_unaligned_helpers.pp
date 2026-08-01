unit u;
interface
type
  tbuf = array[0..15] of byte;
  plongint = ^longint;
procedure raw(var x);
function readl(var b : tbuf; i : longint) : longint;
procedure run(var b : tbuf; i : longint; v : longint; var p : plongint);
implementation
procedure raw(var x);
begin
end;
function readl(var b : tbuf; i : longint) : longint;
begin
  readl := longint(unaligned(plongint(@b[i])^));
end;
procedure run(var b : tbuf; i : longint; v : longint; var p : plongint);
begin
  longint(unaligned(plongint(@b[i])^)) := v;
  inc(longint(unaligned(plongint(@b[i])^)), v);
  p := @longint(unaligned(plongint(@b[i])^));
  raw(longint(unaligned(plongint(@b[i])^)));
end;
end.
