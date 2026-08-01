unit u;
interface
type
  tcgpara = record value : integer; end;
  pcgpara = ^tcgpara;
  thlcg = object
    function call_system_proc(list : pointer; const name : string; const paras : array of pcgpara; force : pointer) : integer; overload;
    function call_system_proc(list : pointer; pd : pointer; const paras : array of pcgpara; force : pointer) : integer; overload;
  end;
procedure demo;
implementation
function thlcg.call_system_proc(list : pointer; const name : string; const paras : array of pcgpara; force : pointer) : integer;
begin
  call_system_proc := 0;
end;
function thlcg.call_system_proc(list : pointer; pd : pointer; const paras : array of pcgpara; force : pointer) : integer;
begin
  call_system_proc := 0;
end;
procedure demo;
var h : thlcg; list, pd : pointer; p1, p2 : tcgpara; r : integer;
begin
  r := h.call_system_proc(list, 'fpc_iocheck', [], nil);
  r := h.call_system_proc(list, pd, [@p1, @p2], nil);
end;
end.
