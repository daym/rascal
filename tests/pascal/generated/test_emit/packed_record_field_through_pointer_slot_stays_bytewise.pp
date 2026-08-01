unit u;
interface
type
  tpacked = packed record
    w : word;
  end;
  ppacked = ^tpacked;
  pslot = ^ppacked;
  pword = ^word;
procedure raw(var x);
function readw(slot : pslot) : word;
procedure run(slot : pslot; n : word; var p : pword);
implementation
procedure raw(var x);
begin
end;
function readw(slot : pslot) : word;
begin
  readw := slot^.w;
end;
procedure run(slot : pslot; n : word; var p : pword);
begin
  slot^.w := n;
  inc(slot^.w, n);
  p := @slot^.w;
  raw(slot^.w);
end;
end.
