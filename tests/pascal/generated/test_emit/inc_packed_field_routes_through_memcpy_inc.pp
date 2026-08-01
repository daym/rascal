unit u;
interface
type tdir = packed record name_ord : word; end;
procedure run(var p : tdir; n : word);
implementation
procedure run(var p : tdir; n : word);
begin
  inc(p.name_ord, n);
end;
end.
