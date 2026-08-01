unit u;
interface
type
  tinlininginfo = class
    flags : integer;
  end;
  tabstractprocdef = class
  end;
  tprocdef = class(tabstractprocdef)
    inlininginfo : tinlininginfo;
  end;
function getflags(pd : tabstractprocdef) : integer;
implementation
function getflags(pd : tabstractprocdef) : integer;
begin
  getflags := (pd as tprocdef).inlininginfo.flags;
end;
end.
