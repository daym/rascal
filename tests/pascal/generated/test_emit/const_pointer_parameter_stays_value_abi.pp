unit u;
interface
procedure take(const p : pointer);
function pick : pointer;
implementation
procedure take(const p : pointer);
begin
end;
function pick : pointer;
begin
  pick := nil;
end;
procedure demo;
begin
  take(pick);
end;
end.
