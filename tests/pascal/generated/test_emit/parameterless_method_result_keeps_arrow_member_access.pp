unit u;
interface
type
  tprocdef = class
    mangledname : string;
  end;
  tprocsym = class
    function first_procdef : tprocdef;
  end;
function getname(ps : tprocsym) : string;
implementation
function tprocsym.first_procdef : tprocdef;
begin
  first_procdef := nil;
end;
function getname(ps : tprocsym) : string;
begin
  getname := ps.first_procdef.mangledname;
end;
end.
