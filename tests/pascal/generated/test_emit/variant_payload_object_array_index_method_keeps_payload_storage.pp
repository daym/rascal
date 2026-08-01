unit u;
interface
type
  tderef = object
    dataidx : longint;
    procedure reset;
  end;
  tderefs = array[1..3] of tderef;
  tview = record
    case byte of
      0 : (items : tderefs);
      1 : (other : longint);
  end;
procedure run(var view : tview; i : longint);
implementation
procedure tderef.reset;
begin
  dataidx := 0;
end;
procedure run(var view : tview; i : longint);
begin
  view.items[i].reset;
end;
end.
