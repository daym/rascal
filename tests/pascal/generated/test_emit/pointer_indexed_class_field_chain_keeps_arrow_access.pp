unit u;
interface
type
  tmodule = class
    modulename : string;
  end;
  tderefmaprec = record
    u : tmodule;
  end;
  pderefmap = ^tderefmaprec;
function getname(m : pderefmap) : string;
implementation
function getname(m : pderefmap) : string;
begin
  getname := m[0].u.modulename;
end;
end.
