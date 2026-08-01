unit u;
interface
type tconvert = longint;
procedure run;
implementation
procedure run;
var result_value : longint;
  function tconvert(value : longint) : longint;
  begin
    tconvert := value;
  end;
begin
  result_value := tconvert(4);
end;
end.
