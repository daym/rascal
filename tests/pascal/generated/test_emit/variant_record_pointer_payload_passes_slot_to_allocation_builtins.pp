unit u;
interface
type
  pint = ^longint;
  tview = record
    case tag : longint of
      0 : (item : pint);
      1 : (text : pchar);
  end;
procedure run(var view : tview; size : longint);
implementation
procedure run(var view : tview; size : longint);
begin
  new(view.item);
  getmem(view.text, size);
end;
end.
