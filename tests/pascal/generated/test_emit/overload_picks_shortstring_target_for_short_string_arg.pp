unit u;
interface
type tidstring = string[127];
function upper(const s : string) : string;
function upper(const s : ansistring) : ansistring;
procedure run(const id : tidstring);
implementation
function upper(const s : string) : string;
begin
  upper := s;
end;
function upper(const s : ansistring) : ansistring;
begin
  upper := s;
end;
procedure run(const id : tidstring);
begin
  upper(id);
end;
end.
