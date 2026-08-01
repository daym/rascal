unit u;
interface
type
  tbase = class
    function getit : pointer; virtual;
  end;
  tchild = class(tbase)
    function getit : pointer; override;
  end;
implementation
function tbase.getit : pointer;
begin
  getit := nil;
end;
function tchild.getit : pointer;
begin
  result := inherited;
end;
end.
