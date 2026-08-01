unit u;
interface
type
  TBase = class
    function MakeRegSize(list : pointer; reg, size : longint) : longint; virtual;
  end;
  TChild = class(TBase)
  protected
    function MakeRegSize(reg, size : longint) : longint; overload;
  public
    procedure Load(list : pointer; var reg : longint; size : longint);
  end;
implementation
function TBase.MakeRegSize(list : pointer; reg, size : longint) : longint;
begin
  MakeRegSize := reg;
end;
function TChild.MakeRegSize(reg, size : longint) : longint;
begin
  MakeRegSize := reg;
end;
procedure TChild.Load(list : pointer; var reg : longint; size : longint);
begin
  reg := MakeRegSize(list, reg, size);
end;
end.
