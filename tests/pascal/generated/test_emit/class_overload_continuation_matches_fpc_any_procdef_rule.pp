unit u;
interface
type
  TBase = class
    procedure Pick(i : longint); overload;
  end;
  TChild = class(TBase)
    procedure Pick(s : string); overload;
    procedure Pick(b : boolean);
  end;
implementation
end.
