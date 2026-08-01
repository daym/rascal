unit u;
interface
type
  tview = record
    case byte of
      0 : (w : word);
      1 : (i : longint);
  end;
function readw(var view : tview) : word;
implementation
function readw(var view : tview) : word;
begin
  readw := unaligned(view.w);
end;
end.
