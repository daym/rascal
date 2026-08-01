unit u;
interface
type
  tbuf = array[0..15] of byte;
  pword = ^word;
procedure raw(var x);
procedure run(var b : tbuf; i : longint; var p : pword);
implementation
procedure raw(var x);
begin
end;
procedure run(var b : tbuf; i : longint; var p : pword);
begin
  p := @unaligned(pword(@b[i])^);
  raw(unaligned(pword(@b[i])^));
end;
end.
