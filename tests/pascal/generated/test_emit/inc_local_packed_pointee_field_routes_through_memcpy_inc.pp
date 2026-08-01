unit u;
interface
procedure run;
implementation
procedure run;
type
  tdir = packed record name_ord : word; end;
  pdir = ^tdir;
var
  p : pdir;
  n : word;
begin
  inc(p^.name_ord, n);
end;
end.
