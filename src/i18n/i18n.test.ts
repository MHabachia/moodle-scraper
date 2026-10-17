import {describe, expect, it} from 'vitest';
import {de} from './de';
import {en} from './en';
import {interpolate, resolveLanguage, translate} from './index';

describe('dictionaries', () => {
  const keys = Object.keys(de) as (keyof typeof de)[];

  it('english has exactly the german keys', () => {
    expect(Object.keys(en).sort()).toEqual([...keys].sort());
  });

  it('placeholders match between languages', () => {
    const ph = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join(',');
    for (const k of keys) expect(ph(en[k]), `Platzhalter in "${k}"`).toBe(ph(de[k]));
  });

  it('no empty texts', () => {
    for (const k of keys) {
      expect(de[k].trim(), k).not.toBe('');
      expect(en[k].trim(), k).not.toBe('');
    }
  });
});

describe('translate', () => {
  it('interpolates params and keeps unknown placeholders', () => {
    expect(interpolate('{a} und {b}', {a: 1})).toBe('1 und {b}');
    expect(translate('de', 'action.selectedMany', {n: 3})).toBe('3 Kurse ausgewählt');
    expect(translate('en', 'action.selectedMany', {n: 3})).toBe('3 courses selected');
  });

  it('resolves language from setting or system', () => {
    expect(resolveLanguage('auto', 'de-DE')).toBe('de');
    expect(resolveLanguage('auto', 'en-US')).toBe('en');
    expect(resolveLanguage('auto', 'fr')).toBe('en');
    expect(resolveLanguage('de', 'en-US')).toBe('de');
    expect(resolveLanguage('en', 'de-DE')).toBe('en');
  });
});
