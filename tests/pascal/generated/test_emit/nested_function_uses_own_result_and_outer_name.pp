unit u;
interface
function outer : integer;
implementation
function outer : integer;
  function inner : boolean;
  begin
    Result := false;
    outer := 123;
  end;
begin
  inner;
end;
end.
