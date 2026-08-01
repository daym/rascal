unit u;
interface
type
  tbase = class
  private
    fcount : longint;
    function getname : string;
  public
    property count : longint read fcount write fcount;
    property name : string read getname;
  end;
  tchild = class(tbase)
    procedure bump;
  end;
implementation
function tbase.getname : string;
begin
  getname := '';
end;
procedure tchild.bump;
begin
  count := count + 1;
  if name <> '' then begin end;
end;
end.
