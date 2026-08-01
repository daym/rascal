unit u;
interface
type
  TList = class
  end;
  TBase = class(TList)
    function clone(L : TList) : TBase; virtual;
  end;
  TBaseClass = class of TBase;
implementation
function TBase.clone(L : TList) : TBase;
begin
  result := TBaseClass(classtype).Create(L);
end;
end.
