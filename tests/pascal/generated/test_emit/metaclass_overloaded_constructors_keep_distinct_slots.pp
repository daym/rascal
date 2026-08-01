unit u;
interface
type
  tnode = class
    constructor create; overload;
    constructor create(n : integer); overload;
  end;
  tnodeclass = class of tnode;
var
  cls : tnodeclass;
  a : tnode;
  b : tnode;
implementation
constructor tnode.create;
begin
end;
constructor tnode.create(n : integer);
begin
end;
begin
  a := cls.create;
  b := cls.create(7);
end.
