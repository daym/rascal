unit u;
interface
function nexttoken(value : pointer) : pointer;
implementation
function nexttoken(value : pointer) : pointer;
begin
  nexttoken := pointer(ptruint(value) + 1);
end;
end.
