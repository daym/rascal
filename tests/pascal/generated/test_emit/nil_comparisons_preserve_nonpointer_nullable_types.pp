unit u;
interface
type tcallback = procedure of object;
procedure run;
implementation
procedure run;
var callback : tcallback;
    values : array of byte;
    b : boolean;
begin
  b := callback <> nil;
  b := values <> nil;
end;
end.
