unit u;
interface
type
  tarr = array[1..3] of char;
  ptarr = ^tarr;
  tview = record
    case tag : longint of
      0 : (items : tarr);
      1 : (other : longint);
  end;
procedure run(var view : tview; var pc : pchar; var pa : ptarr);
implementation
procedure run(var view : tview; var pc : pchar; var pa : ptarr);
begin
  pc := @view.items;
  pa := @view.items;
end;
end.
