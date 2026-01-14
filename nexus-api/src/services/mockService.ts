import { v4 as uuidv4 } from 'uuid';

export const generateMock = (type: string): string | number | boolean => {
  const t = type.toLowerCase().trim();
  
  switch (t) {
    case 'uuid':
      return uuidv4();
    case 'email':
      return `user_${Math.floor(Math.random() * 10000)}@example.com`; // Valid domain for aesthetics
    case 'username':
      return `user_${Math.floor(Math.random() * 10000)}`;
    case 'firstname':
      const firsts = ['James', 'Mary', 'John', 'Patricia', 'Robert', 'Jennifer', 'Michael', 'Linda', 'David', 'Sarah'];
      return firsts[Math.floor(Math.random() * firsts.length)];
    case 'lastname':
      const lasts = ['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis', 'Rodriguez', 'Martinez'];
      return lasts[Math.floor(Math.random() * lasts.length)];
    case 'fullname':
      return `${generateMock('firstname')} ${generateMock('lastname')}`;
    case 'job':
    case 'title':
      const jobs = ['Leader', 'Developer', 'Designer', 'Manager', 'Analyst', 'Engineer', 'Architect'];
      return jobs[Math.floor(Math.random() * jobs.length)];
    case 'company':
      const companies = ['Nexus', 'Google', 'Amazon', 'Meta', 'Netflix', 'Tesla', 'Microsoft'];
      return companies[Math.floor(Math.random() * companies.length)];
    case 'avatar':
      return `https://i.pravatar.cc/128?u=${Math.floor(Math.random() * 1000)}`;
    case 'number':
    case 'randomint':
      return Math.floor(Math.random() * 1000);
    case 'boolean':
      return Math.random() < 0.5;
    case 'timestamp':
      return Date.now();
    case 'date':
      return new Date().toISOString().split('T')[0];
    case 'city':
      const cities = ['New York', 'London', 'Tokyo', 'Paris', 'Berlin', 'Sydney', 'Toronto', 'Hanoi', 'Saigon'];
      return cities[Math.floor(Math.random() * cities.length)];
    default:
      return `{{$mock.${type}}}`; // Return as is if unknown
  }
};

export const isMockKey = (key: string) => key.startsWith('$mock.') || key.startsWith('$random');

export const resolveMock = (key: string) => {
    if (key.startsWith('$mock.')) {
        return generateMock(key.replace('$mock.', ''));
    }
    if (key === '$randomInt') return generateMock('number');
    if (key === '$uuid') return generateMock('uuid');
    if (key === '$timestamp') return generateMock('timestamp');
    return key;
};