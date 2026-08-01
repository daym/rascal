unit u;
interface
type
  thandler = class
    type
      ttemps = record value : integer; end;
      tstate = record code : integer; end;
    class procedure start(var t : ttemps); virtual;
    class procedure finish(const t : ttemps; out s : tstate); virtual;
  end;
implementation
class procedure thandler.start(var t : ttemps);
begin
  t.value := 1;
end;
class procedure thandler.finish(const t : ttemps; out s : tstate);
begin
  s.code := t.value;
end;
end.
