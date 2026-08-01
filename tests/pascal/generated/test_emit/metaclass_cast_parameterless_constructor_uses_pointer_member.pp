unit u;
interface
type
  titem = class
    constructor create;
    function copy : titem;
  end;
  titemclass = class of titem;
implementation
constructor titem.create;
begin
end;
function titem.copy : titem;
begin
  result := titemclass(classtype).create;
end;
end.
