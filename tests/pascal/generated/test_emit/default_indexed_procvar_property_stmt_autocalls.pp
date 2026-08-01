unit u;
interface
type
  tproc = procedure;
  tbox = class
    function getitem(index : longint) : tproc;
    property items[index : longint] : tproc read getitem; default;
  end;
procedure demo(box : tbox);
implementation
function tbox.getitem(index : longint) : tproc;
begin
  getitem := nil;
end;
procedure demo(box : tbox);
begin
  box[1];
  box[2]();
end;
end.
