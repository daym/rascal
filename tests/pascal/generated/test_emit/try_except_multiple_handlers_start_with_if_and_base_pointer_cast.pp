unit u;
interface
type
  efoo = class(exception)
  end;
  ebar = class(exception)
  end;
function demo : boolean;
implementation
function demo : boolean;
begin
  try
    raise efoo.create;
  except
    on a : efoo do
      Result := a <> nil;
    on b : exception do
      Result := b <> nil;
  end;
end;
end.
