unit u;
interface
type
  tarr = array[1..3] of longint;
  tview = record
    case tag : longint of
      0 : (items : tarr);
      1 : (other : longint);
  end;
procedure run(var view : tview; i : longint; value : longint);
implementation
procedure run(var view : tview; i : longint; value : longint);
begin
  view.items[i] := value;
end;
end.
