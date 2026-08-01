unit u;
interface
type
  tcb = procedure(x : integer) of object;
  tobj = object
    cb : tcb;
    procedure fire(x : integer);
    procedure setcb;
  end;
implementation
procedure tobj.fire(x : integer);
begin
end;
procedure tobj.setcb;
begin
  cb := fire;
end;
end.
