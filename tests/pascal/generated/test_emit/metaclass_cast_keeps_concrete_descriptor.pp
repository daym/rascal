unit u;
interface
type
  tbase = class
    constructor create;
  end;
  tchild = class(tbase)
  end;
  tbaseclass = class of tbase;
  tchildclass = class of tchild;
var
  basecls : tbaseclass;
  childcls : tchildclass;
implementation
constructor tbase.create;
begin
end;
begin
  basecls := tchild;
  childcls := tchildclass(basecls);
end.
