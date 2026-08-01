unit u;
interface
type
  tbytes = array of byte;
  tinfo = record
    values : tbytes;
  end;
  pinfo = ^tinfo;
procedure callback(arg : pointer);
implementation
procedure callback(arg : pointer);
begin
  setlength(pinfo(arg)^.values, 1);
end;
end.
