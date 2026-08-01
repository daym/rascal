unit u;
interface
type
  efoo = class(exception)
  end;
procedure demo;
implementation
procedure demo;
begin
  raise efoo.create at get_caller_addr(get_frame), get_caller_frame(get_frame);
end;
end.
