unit u;
interface
function upper(const c : char) : char;
function upper(const s : string) : string;
function upper(const s : ansistring) : ansistring;
procedure run(p : pchar);
implementation
function upper(const c : char) : char;
begin
  upper := c;
end;
function upper(const s : string) : string;
begin
  upper := s;
end;
function upper(const s : ansistring) : ansistring;
begin
  upper := s;
end;
procedure run(p : pchar);
begin
  upper(p);
end;
end.
