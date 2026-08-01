unit u;
interface
type
  tproc = procedure;
  tbox = class
  private
    ffoo : tproc;
    function getfoo : tproc;
  public
    property foo : tproc read getfoo write ffoo;
  end;
procedure demo(box : tbox; p : tproc);
implementation
function tbox.getfoo : tproc;
begin
  getfoo := ffoo;
end;
procedure demo(box : tbox; p : tproc);
begin
  box.foo;
  box.foo();
  p := box.foo;
end;
end.
