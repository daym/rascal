unit u;
interface
type
  tbox = class
  private
    procedure setval(v : longint);
  public
    property val : longint write setval;
  end;
procedure write_it(b : tbox);
implementation
procedure tbox.setval(v : longint);
begin
end;
procedure write_it(b : tbox);
begin
  b.val := 42;
end;
end.
