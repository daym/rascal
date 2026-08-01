unit u;
interface
uses sysutils;
procedure message1(w : longint; const s1 : string);
function tostr(i : longint) : string;
procedure demo;
implementation
procedure message1(w : longint; const s1 : string);
begin
end;
function tostr(i : longint) : string;
begin
  tostr := '';
end;
procedure demo;
begin
  try
    raise exception.create('x');
  except
    on E : EOSError do
      message1(1, tostr(E.ErrorCode));
  end;
end;
end.
