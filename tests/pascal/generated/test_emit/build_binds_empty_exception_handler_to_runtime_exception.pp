unit u;
interface
procedure demo;
implementation
procedure demo;
begin
  try
  except
    on exception do
      exit;
  end;
end;
end.
