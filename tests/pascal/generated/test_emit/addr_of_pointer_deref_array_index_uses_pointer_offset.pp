unit u;
interface
type
  tarr = array[0..9] of char;
  ptarr = ^tarr;
procedure run(p : ptarr; i : longint; var outp : pchar);
implementation
procedure run(p : ptarr; i : longint; var outp : pchar);
begin
  outp := @p^[i];
end;
end.
