unit u;
interface
type
  pint = ^longint;
  tarr = array[1..3] of longint;
  tview = record
    case tag : longint of
      0 : (items : tarr);
      1 : (other : longint);
  end;
procedure raw(var x);
function readitem(var view : tview; i : longint) : longint;
function addritem(var view : tview; i : longint) : pint;
procedure passitem(var view : tview; i : longint);
implementation
procedure raw(var x);
begin
end;
function readitem(var view : tview; i : longint) : longint;
begin
  readitem := view.items[i];
end;
function addritem(var view : tview; i : longint) : pint;
begin
  addritem := @view.items[i];
end;
procedure passitem(var view : tview; i : longint);
begin
  raw(view.items[i]);
end;
end.
