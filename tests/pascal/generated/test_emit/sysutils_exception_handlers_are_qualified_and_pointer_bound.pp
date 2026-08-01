unit u;
interface
uses sysutils;
function demo : boolean;
implementation
function demo : boolean;
begin
  try
    raise exception.create('x');
  except
    on EOutOfMemory do
      Result := true;
    on e : EInOutError do
      Result := e.message <> '';
  end;
end;
end.
