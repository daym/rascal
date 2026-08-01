unit u;
interface
type
  tinfo = record
    line : longint;
  end;
  pinfo = ^tinfo;
  pai = ^tai;
  tai = object
    fileinfo : tinfo;
  end;
function last_fileinfo(last : pai) : pinfo;
implementation
function last_fileinfo(last : pai) : pinfo;
begin
  last_fileinfo := @last^.fileinfo;
end;
end.
