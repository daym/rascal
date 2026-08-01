unit u;
interface
type
  tbase = class
    function getcopy : tbase; virtual;
  end;
  tchild = class(tbase)
    function getcopy : tbase; virtual; reintroduce;
  end;
implementation
function tbase.getcopy : tbase; begin getcopy := nil; end;
function tchild.getcopy : tbase; begin getcopy := nil; end;
end.
