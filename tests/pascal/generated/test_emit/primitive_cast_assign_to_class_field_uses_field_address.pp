unit u;
interface
type
  TKind = (ka, kb);
  TObj = class
  public
    IType : TKind;
  end;
procedure load(o : TObj; b : byte);
implementation
procedure load(o : TObj; b : byte);
begin
  byte(o.IType) := b;
end;
end.
