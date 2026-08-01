unit u;
interface
type
  tdouble_result = record value : longint; end;
  textended_result = record value : longint; end;
function pick(v : double) : tdouble_result; overload;
function pick(v : extended) : textended_result; overload;
procedure take(v : textended_result);
procedure run(a, b : extended);
implementation
function pick(v : double) : tdouble_result;
begin
  result.value := 1;
end;
function pick(v : extended) : textended_result;
begin
  result.value := 2;
end;
procedure take(v : textended_result);
begin
end;
procedure run(a, b : extended);
begin
  take(pick(a / b));
end;
end.
