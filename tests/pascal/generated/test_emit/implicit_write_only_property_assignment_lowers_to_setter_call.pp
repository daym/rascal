unit u;
interface
type
  tbox = class
  private
    procedure setval(v : longint);
    procedure putslot(i : longint; v : longint);
  public
    property val : longint write setval;
    property slots[i : longint] : longint write putslot;
    procedure write_it;
  end;
implementation
procedure tbox.setval(v : longint);
begin
end;
procedure tbox.putslot(i : longint; v : longint);
begin
end;
procedure tbox.write_it;
begin
  val := 42;
  slots[3] := 9;
end;
end.
