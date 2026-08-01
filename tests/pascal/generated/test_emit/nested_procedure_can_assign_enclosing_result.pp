unit u;
interface
function demo : integer;
implementation
function demo : integer;
  procedure setit(v : integer);
  begin
    Result := v;
  end;
begin
  setit(3);
end;
end.
