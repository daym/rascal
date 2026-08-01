unit u;
interface
type
  tbase = class
    constructor create;
  end;
  tchild = class(tbase)
    constructor create;
  end;
  tchildclass = class of tchild;
var
  cls : tchildclass;
  inst : tchild;
implementation
constructor tbase.create;
begin
end;
constructor tchild.create;
begin
end;
begin
  inst := cls.create;
end.
