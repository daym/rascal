unit u;
interface
type
  tbase = class end;
  tbox = class(tbase)
  private
    procedure setval(v : longint);
  public
    property val : longint write setval;
  end;
procedure write_it(b : tbase);
implementation
procedure tbox.setval(v : longint);
begin
end;
procedure write_it(b : tbase);
begin
  tbox(b).val := 42;
end;
end.
