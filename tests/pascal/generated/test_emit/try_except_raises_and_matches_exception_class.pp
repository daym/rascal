unit u;
interface
type
  efoo = class(exception)
  end;
function demo : boolean;
implementation
function demo : boolean;
begin
  try
    raise efoo.create;
  except
    on e : efoo do
      Result := e <> nil;
  end;
end;
end.
