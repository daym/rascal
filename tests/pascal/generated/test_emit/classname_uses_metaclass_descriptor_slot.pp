unit u;
interface
type
  titem = class
  end;
  titemclass = class of titem;
function inst_name(x : titem) : shortstring;
function meta_name(c : titemclass) : shortstring;
function direct_name : shortstring;
implementation
function inst_name(x : titem) : shortstring;
begin
  inst_name := x.classname;
end;
function meta_name(c : titemclass) : shortstring;
begin
  meta_name := c.classname;
end;
function direct_name : shortstring;
begin
  direct_name := titem.classname;
end;
end.
