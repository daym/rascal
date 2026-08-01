unit u;
interface
type
  tview = record
    case tag : longint of
      0 : (text : pchar);
      1 : (other : longint);
  end;
procedure run(var view : tview; size : longint);
implementation
procedure getmem(var p : pchar; size : longint);
begin
end;
procedure run(var view : tview; size : longint);
begin
  getmem(view.text, size);
end;
end.
