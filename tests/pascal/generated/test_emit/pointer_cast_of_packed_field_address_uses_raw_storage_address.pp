unit u;
interface
type
  pbyte = ^byte;
  trec = packed record
    value : longint;
  end;
procedure run(var r : trec; var p : pbyte);
implementation
procedure run(var r : trec; var p : pbyte);
begin
  p := pbyte(@r.value);
end;
end.
