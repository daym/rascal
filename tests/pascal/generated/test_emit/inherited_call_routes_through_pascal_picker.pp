unit u;
interface
type
  tbase = class
    procedure foo(s : shortstring); overload; virtual;
    procedure foo(s : ansistring); overload; virtual;
  end;
  tderived = class(tbase)
    procedure foo(s : shortstring); override;
  end;
implementation
procedure tbase.foo(s : shortstring); begin end;
procedure tbase.foo(s : ansistring); begin end;
procedure tderived.foo(s : shortstring);
var hs : shortstring;
begin
  inherited foo(hs);
end;
end.
