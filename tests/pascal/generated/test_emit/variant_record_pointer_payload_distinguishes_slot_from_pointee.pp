unit u;
interface
type
  pint = ^longint;
  tview = record
    case tag : longint of
      0 : (item : pint);
      1 : (value : longint);
  end;
procedure touch(var p : pint);
procedure setslot(var view : tview; p : pint);
procedure setpointed(var view : tview; v : longint);
function addrpointed(var view : tview) : pint;
implementation
procedure touch(var p : pint);
begin
end;
procedure setslot(var view : tview; p : pint);
begin
  view.item := p;
  touch(view.item);
end;
procedure setpointed(var view : tview; v : longint);
begin
  view.item^ := v;
end;
function addrpointed(var view : tview) : pint;
begin
  addrpointed := @view.item^;
end;
end.
