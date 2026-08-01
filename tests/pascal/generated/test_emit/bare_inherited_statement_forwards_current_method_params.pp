unit u;
interface
type
  tbase = class
    procedure touch(const name: string); virtual;
  end;
  tchild = class(tbase)
    procedure touch(const name: string); override;
  end;
implementation
procedure tbase.touch(const name: string);
begin
end;
procedure tchild.touch(const name: string);
var
  local: string;
begin
  local := name;
  inherited;
end;
end.
