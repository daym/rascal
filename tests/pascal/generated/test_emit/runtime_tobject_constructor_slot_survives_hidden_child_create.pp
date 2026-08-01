unit u;
interface
type
  tchild = class
    constructor create(n : integer);
  end;
  tbaseclass = class of TObject;
var
  cls : tbaseclass;
  inst : TObject;
implementation
constructor tchild.create(n : integer);
begin
end;
begin
  cls := tchild;
  inst := cls.create;
end.
