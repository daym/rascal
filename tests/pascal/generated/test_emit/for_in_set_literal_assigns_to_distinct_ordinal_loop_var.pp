unit u;
interface
type
  tsuperregister = type word;
const
  rs_s1 = $20;
  rs_s3 = $22;
procedure p;
implementation
procedure p;
var i : tsuperregister; total : longint;
begin
  total := 0;
  for i in [rs_s1, rs_s3] do
    total := total + ord(i);
end;
end.
